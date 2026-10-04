import QtQuick
import QtQuick.Dialogs
import QtQuick.Layouts
import platitude
import platitude.ui

// The settings screen's `Application` category: what Platitude GG keeps in its own settings file (規約 §設定の画面).
// One file per category, since a category is one place values are stored.
ColumnLayout {
    id: pane

    /// The page the avatar candidates are read off (its repository's authors).
    property var curPage: null

    /// The author an avatar's badge was pressed on, handed down by the screen for the entry.
    property string prefillName: ""
    property string prefillEmail: ""

    /// Enter in one of the number fields. The way out belongs to the screen, so it is asked for.
    signal accepted()

    /// Stands in for the pointer on one row's Remove, which headless cannot inject.
    property int pointedAtRow: -1

    /// What the avatar verbs (finished by `WindowAutoActDriver`) wait on — the list's output side.
    readonly property int autoAvatarRows: avatarRepeater.count
    readonly property bool autoAvatarComboOpen: avatarWho.popup.opened
    /// Whether this category is showing, for the verb that presses the rail — reading back the screen's `category`
    /// would report the run's own press.
    readonly property bool autoAppShown: fetchField.visible
    /// The version chip can be grabbed (`CodeChip.grabbable`), which a picture cannot show.
    readonly property bool autoVersionGrabbed: gitVersionChip.grabbed
    function autoAvatarRowLit(at) {
        const row = avatarRepeater.itemAt(at)
        return !!row && row.lit
    }
    function autoAvatarRowPainted(at) {
        const row = avatarRepeater.itemAt(at)
        return !!row && row.pictureReady()
    }
    /// Through the press's own door (`AppCombo.pressField`), including its turn of the loop before the list.
    function autoAvatarOfferCombo() {
        avatarWho.pressField()
    }
    /// Typing into that box and Enter, which the offscreen platform does not deliver: the text goes where typing
    /// writes it (`AppCombo.wanted`, as `focusPrefill` does), and Enter is the box's own `accepted` (`tst_appcombo`).
    function autoTypeAvatarWho(who) {
        avatarWho.wanted = who
        avatarWho.editText = who
    }
    function autoEnterAvatarWho() {
        avatarWho.accepted()
    }
    /// `visible`, not `opened`: a `FileDialog` is a platform window, not a `Popup` — and it is in neither PNG.
    readonly property bool autoAvatarPickerOpen: avatarPicker.visible
    /// Runs the row's hold to its end. False where the list has no such row yet, or its button takes no press yet,
    /// so the caller waits for it.
    function autoAvatarHoldRemove(at) {
        const row = avatarRepeater.itemAt(at)
        return row !== null && row.holdRemove()
    }

    /// Assignments as the settings file holds them (`AppBackend.avatars` — `{email, name, url}`). A picture filed
    /// under no name is shown by its address.
    readonly property var assigned: AppBackend.avatars.map(assignment => ({
        email: assignment.email,
        name: assignment.name !== "" ? assignment.name : assignment.email,
        url: assignment.url
    }))

    /// Who the entry offers, read once as the screen opens — bound, it would reorder under an open popup as the
    /// graph moves.
    property var authorChoices: []
    function readAuthorChoices() {
        // `GraphModel.author_choices` dedupes, sorts and formats, with the prefill so its address shows once.
        pane.authorChoices = pane.curPage
                           ? pane.curPage.pageGraph.authorChoices(pane.prefillName, pane.prefillEmail)
                           : pane.prefillEmail !== ""
                             ? [pane.prefillName + " <" + pane.prefillEmail + ">"] : []
    }

    /// The address and name the picker files under, split out of the entry: `Name <address>`, or either half typed
    /// alone (`platitude_core::trailers::split_identity`).
    readonly property string chosenEmail: GitFacts.identityEmailOf(avatarWho.wanted)
    readonly property string chosenName: GitFacts.identityNameOf(avatarWho.wanted)
    /// The one body the button and the box's Enter share, guarded as the button is: a picker opened over nobody
    /// would file against an empty address.
    function openAvatarPicker() {
        if (pane.chosenEmail === "")
            return
        avatarPicker.open()
    }

    /// The version git printed, in git's own spelling — the chip at the head of the line. Empty where git printed
    /// none.
    readonly property string gitPathChip:
        AppBackend.gitPathState === "ok" || AppBackend.gitPathState === "old"
        ? "git " + AppBackend.gitPathVersion : ""
    /// What the git at the path said, as one sentence — empty where the chip says it all. The words live here
    /// (rules-refs/app-ui.md「Rust に文言を置かない」); `version::Probe` sends which answer it was. The two `missing`
    /// sentences differ: an empty box means the git on PATH, a written path is a file that is not there.
    readonly property string gitPathWord: {
        // The version chip is the warning's subject. Two whole wordings rather than a joined-on clause
        // (規約 §設定の画面).
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
            // The chip alone is the whole answer.
            return ""
        case "old":
            // The `OLD GIT` badge's words, less the version the chip carries (規約 §git が無い時・古い時).
            return qsTr("is older than the %1 this app is built for.").arg(AppBackend.minimumGit)
        case "missing":
            return AppBackend.gitPath === "" ? qsTr("git was not found on PATH.")
                                             : qsTr("Nothing to run at that path.")
        case "failed":
            return AppBackend.gitPathError
        }
        return ""
    }
    /// `warning` while a restart is offered or git is old; `danger` where git did not answer; otherwise the help
    /// text's ink — a version that came back is ordinary (規約 §状態).
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

    /// Whether the button stands, frozen for the length of a press (規約 §設定の画面「姿は押した瞬間に凍る」): the
    /// offer moves on git's clock and could take the button from under a hand mid-hold; frozen, the press finishes and
    /// `restart_now` decides whether it still means anything. The `Binding` below keeps it live and lets go during a
    /// press (`RestoreNone` is the freeze).
    property bool gitButtonRestarts: AppBackend.gitPathOffersRestart
    /// `warning` for a git that answers, `danger` where it is below the supported minimum — the button alone goes
    /// red; the line stays `warning`, being old a fact about the git.
    readonly property color gitButtonTone:
        AppBackend.gitPathState === "old" ? Theme.danger : Theme.warning
    /// A press is under way: the pointer's `down`, or the keyboard's climbing fill (`HoldDriver.pressKey`).
    readonly property bool gitButtonPressing: gitButton.down || gitButton.holdProgress > 0
    Binding {
        target: pane
        property: "gitButtonRestarts"
        value: AppBackend.gitPathOffersRestart
        when: !pane.gitButtonPressing
        restoreMode: Binding.RestoreNone
    }

    /// The git box's state in one reading — shown, stored, and what the binary said — none of it readable off a
    /// picture. Automation-only, the same exposure as `GraphPane.view` (rules-refs/app-ui.md).
    function gitPathTally() {
        // The six flags stay together, ahead of `state=`: each verb-table row claims a run of them as one substring
        // (`verify::verbs`). `named` is a placeholder there at all — `native` alone passes an empty one. `native` is a
        // backslash in what the box shows, its value or its placeholder (規約 §パスの区切り). `answered` folds `ok` and
        // `old` so the supported minimum and the newest git read alike.
        return "shown=" + gitPathField.text + " stored=" + AppBackend.gitPath
             + " named=" + (gitPathField.placeholderText !== "")
             + " native=" + (gitPathField.text.includes("\\") || gitPathField.placeholderText.includes("\\"))
             + " answered=" + (AppBackend.gitPathState === "ok" || AppBackend.gitPathState === "old")
             + " offers=" + AppBackend.gitPathOffersRestart
             + " held=" + pane.gitButtonRestarts
             + " restart=" + AppBackend.restartWanted
             + " state=" + AppBackend.gitPathState
             + " version=" + AppBackend.gitPathVersion
             + " inUse=" + AppBackend.gitPathInUse
    }
    /// Opens the chooser beside a git — on a machine with several, the nearest useful start: the box's path, or with
    /// the box empty the git this run spawns.
    function openGitPicker() {
        const near = GitFacts.folderUrlOf(gitPathField.text !== "" ? gitPathField.text
                                                                  : AppBackend.gitPathInUse)
        if (near !== "")
            gitPicker.currentFolder = near
        gitPicker.open()
    }

    /// The chooser's answer, a `file:` URL, put in the box and written — the box's one way in besides the store.
    function takeGitPick(url) {
        gitPathField.text = GitFacts.pickedPath(url)
        pane.applyGitPath()
    }
    /// Answers in the chooser's place, through its door: the box spells a path only as the chooser's answer reads.
    /// Writing `AppBackend.gitPath` straight would pass with the wiring cut (app-ui.md §UI 自動化).
    function autoPickGitPath(url) {
        pane.takeGitPick(url)
    }

    /// Both process boxes, each typed the way a hand leaves a box: the text, then the edit finished. An empty half is
    /// an emptied box: the default / off.
    function autoTypeProcesses(concurrency, copiesSecs) {
        concurrencyField.text = concurrency
        pane.applyConcurrency()
        copiesField.text = copiesSecs
        pane.applyCopies()
    }
    /// What the store holds for that chapter: the count and whether it is this machine's default (the number itself
    /// varies by machine), and the interval in both units — the tick reads milliseconds, the box seconds.
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
        // One `git --version` for the path shown.
        AppBackend.openGitPathScreen()
        pane.readAuthorChoices()
    }
    /// Opened from an avatar, the entry is filled and focus goes to choosing the picture — the only focus this
    /// screen places as it opens (`SettingsDialog.onOpened`).
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

    // Empty is off.
    function applyFetch() {
        AppBackend.setAutoFetchMinutes(fetchField.text === "" ? 0 : Number(fetchField.text))
    }
    // Empty is the default (zero at once is meaningless). Also written from `applyFields`: a validator with a floor
    // rejects an empty string, so `onEditingFinished` never fires for an emptied field.
    function applyConcurrency() {
        AppBackend.setGitConcurrency(concurrencyField.text === "" ? 0 : Number(concurrencyField.text))
    }
    // Empty is never.
    function applyCopies() {
        AppBackend.setCopiesIntervalSecs(copiesField.text === "" ? 0 : Number(copiesField.text))
    }
    // Two controls, one value: the box asks for no window at all (core's `0`); the field answers only while the box
    // is clear, empty being the default. Written from `applyFields` too, as `applyConcurrency` is.
    function applyCommits() {
        AppBackend.setInitialCommits(wholeHistoryBox.checked ? 0
                                     : commitsField.text === "" ? AppBackend.initialCommitsDefault
                                     : Number(commitsField.text))
    }
    // Writing the path also asks the binary. Empty is an answer (the git `PATH` resolves), so nothing is guarded.
    function applyGitPath() {
        AppBackend.setGitPath(gitPathField.text)
    }
    /// Writes every field, for the way out of the screen. The git path is only written — putting it in force is the
    /// button's errand (`AppBackend.applyGitPathNow`): closing a settings screen must not restart the window.
    function applyFields() {
        pane.applyFetch()
        pane.applyConcurrency()
        pane.applyCopies()
        pane.applyCommits()
        pane.applyGitPath()
    }

    Layout.fillWidth: true
    spacing: Theme.spaceXl

    HelpText {
        text: qsTr("Kept by Platitude GG in its own settings file. Nothing here is written to your git configuration.")
    }

    // First, because every other chapter runs through which binary answers (規約 §設定の画面).
    SettingsSection {
        caption: qsTr("GIT EXECUTABLE")
        HelpText {
            // The first sentence is the term of art every other tool names this by (規約 §設定の画面).
            text: qsTr("Path to the git executable this app runs, in every repository. Empty is the one found on PATH, which the box names.")
        }
        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceSm
            // The box is the chooser — a typed path one letter wrong names nothing. It keeps a typing box's ground,
            // this screen's one exception to §選ぶ欄と打つ欄: bare between two grounded boxes it read as disabled.
            // Full width: a path has no fixed length (デザイン規約 §レイアウト初期値).
            FormField {
                id: gitPathField
                Layout.fillWidth: true
                choosing: true
                // What an empty box is read as: the resolved path, since "the git on PATH" is nowhere a reader can
                // look (規約 §設定の画面). Spelled with `/` where it comes in (規約 §パスの区切り).
                placeholderText: AppBackend.gitPathInUse
                onPicking: pane.openGitPicker()
            }
            // The one button puts a git in force, standing only while there is one to put — choosing is the box's
            // errand. `warning` and held: the press costs the window (規約 §設定の画面 / §長押し).
            ActionButton {
                id: gitButton
                visible: pane.gitButtonRestarts
                implicitHeight: Theme.controlHeight
                text: qsTr("Restart to apply")
                tone: pane.gitButtonTone
                // Framed beside a framed box (rules-refs/app-ui.md「肯定側のボタンは枠で名乗り」), in the word's
                // colour to say the press costs something.
                frameColor: pane.gitButtonTone
                holdMs: Metrics.holdMs
                activeFocusOnTab: true
                onHeld: AppBackend.applyGitPathNow()
            }
        }
        // What the git at that path said, below the box: the explanation above, what is true now below
        // (規約 §設定の画面). The version wears the chip (規約 §git 用語のコード表記) — a bare lowercase `git` opening a
        // line reads as a typo. One sentence beside the chip, since a chip cannot flow inside wrapping text (同 §);
        // anything more goes below.
        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceXs
            visible: pane.gitPathChip !== "" || pane.gitPathWord !== ""
            CodeChip {
                id: gitVersionChip
                visible: pane.gitPathChip !== ""
                word: pane.gitPathChip
                tint: pane.gitPathInk
                // The one chip in this window that is a value: what a reader copies into a bug report (規約 §設定の画面).
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
        // What a restart costs (nothing), kept off the chip's one-sentence line. Comes and goes with the button.
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
            // The folder is set on the way in (`openGitPicker`), not bound: a dialog re-homing itself mid-walk would
            // take the reader's place away. No name filter: git has no extension off Windows, a wrapper script is fine,
            // and the version written back says whether the choice was a git.
            onAccepted: pane.takeGitPick(selectedFile.toString())
        }
    }

    // The rest of the category is held while a git waits to be applied (規約 §設定の画面): work set now would go with
    // the window being replaced. `enabled` takes the chapter's words to the disabled step with its boxes (§無効).
    SettingsSection {
        enabled: !AppBackend.gitPathOffersRestart
        caption: qsTr("AUTOMATIC FETCH")
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
                    // On Enter or focus out — per keystroke would pass through "1" on the way to "10".
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

    // Beside the fetch interval: the same kind of answer, about this machine (規約 §設定の画面).
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
                    // Disabled, not cleared, under the whole history (§無効): unchecking gives the number back.
                    enabled: !wholeHistoryBox.checked
                    placeholderText: String(AppBackend.initialCommitsDefault)
                    inputMethodHints: Qt.ImhDigitsOnly
                    // No top: the only ceiling is the property's own type (`session::log_limit`).
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
        // A yes or no, so a box — an empty field already means the default (規約 §設定の画面).
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
            /// The widest row's name, so every address starts on the same x (as `AppMenu.codeColW`). Settled by the
            /// rows' own seat changes: a binding re-runs only on `count`, and a same-size reassignment replaces every
            /// row.
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
                // Enter opens the picker, as the button beside it does (デザイン規約 §アバターを与える) — the git box's
                // Enter rule (`FormField.answerKey`) for a box that also takes typing.
                onSubmitted: pane.openAvatarPicker()
            }
            ActionButton {
                id: chooseAvatar
                implicitHeight: Theme.controlHeight
                // The feature's one word (デザイン規約 §アバターを与える); a bare noun for a kind about to be chosen
                // (§長さ).
                text: qsTr("Choose avatar…")
                // A plain frame in a form row next to a combo (rules-refs/app-ui.md「肯定側のボタンは枠で名乗り」).
                frameColor: Theme.borderDefault
                activeFocusOnTab: true
                enabled: pane.chosenEmail !== ""
                onActivated: pane.openAvatarPicker()
            }
        }
        CardText {
            Layout.fillWidth: true
            visible: AppBackend.avatarErrorKind !== ""
            color: Theme.danger
            pixelSize: Theme.fontSm
            // Our words off core's kind and facts; an OS failure adds its own line under ours (`Words.avatarFailure`).
            text: Words.avatarFailure(AppBackend.avatarErrorKind,
                                      AppBackend.avatarErrorFacts,
                                      AppBackend.avatarErrorSaid)
        }
    }

    FileDialog {
        id: avatarPicker
        title: qsTr("Choose avatar")
        // Patterns from the store so the two cannot drift; the words are ours (rules-refs/app-ui.md「Rust に文言を置かない」).
        // They name the two formats — a category word would be a second name for the avatar — and the formats are
        // pinned by デザイン規約 §アバターを与える, so `avatar::EXTENSIONS` cannot grow without the rules moving first.
        nameFilters: [qsTr("PNG and JPEG (%1)").arg(AppBackend.avatarPatterns)]
        onAccepted: AppBackend.assignAvatar(pane.chosenEmail, pane.chosenName, selectedFile.toString())
    }
}
